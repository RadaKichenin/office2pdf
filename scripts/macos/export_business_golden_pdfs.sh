#!/usr/bin/env bash
set -euo pipefail

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
project_root="$(cd "$script_dir/../.." && pwd)"
corpus_root="$project_root/tests/golden_mocks/business"
stage_root="${1:-$project_root/target/business-golden-office-export}"
containers_root="$HOME/Library/Containers"
# PlistBuddy is the one tool here with no PATH entry to shadow, so the version
# probe takes an override; the script's tests run where Office is not installed.
plist_buddy="${PLIST_BUDDY_BIN:-/usr/libexec/PlistBuddy}"

command -v osascript >/dev/null
command -v pdfunite >/dev/null
command -v pdfinfo >/dev/null

mkdir -p "$stage_root/docx" "$stage_root/pptx" "$stage_root/xlsx-sheets" "$stage_root/xlsx"

container_stages=()

remove_container_stages() {
    local stage
    for stage in ${container_stages+"${container_stages[@]}"}; do
        rm -rf "$stage"
    done
}
trap remove_container_stages EXIT

# Word, PowerPoint and Excel are sandboxed: a source opened from, or a PDF saved
# to, anywhere outside the app's own container costs a per-file "Grant Access"
# dialog, and an unattended run stalls on the first one (#1051, #1082, #1128).
# So each format round-trips through the container of the app that exports it —
# reaching into another app's container prompts just the same — and the PDFs are
# copied back to the caller's stage root afterwards. Only the unsandboxed
# pdfunite/pdfinfo steps below ever touch that stage root.
export_format() {
    local extension="$1" bundle_id="$2" exporter="$3" destination="$4"
    local stage="$containers_root/$bundle_id/Data/business-golden-export"
    local staged_sources="$stage/sources" staged_pdfs="$stage/pdf"

    # A stage left behind by an aborted run is the next run's stale input.
    rm -rf "$stage"
    container_stages+=("$stage")
    mkdir -p "$staged_sources" "$staged_pdfs"

    local -a exporter_args=("$staged_pdfs")
    local source stem staged
    while IFS= read -r -d '' source; do
        stem="$(basename "${source%.$extension}")"
        staged="$staged_sources/$stem.$extension"
        cp "$source" "$staged"
        exporter_args+=("$stem" "$staged")
    done < <(find "$corpus_root/sources/$extension" -maxdepth 1 -type f -name "*.$extension" -print0 | sort -z)

    if [[ ${#exporter_args[@]} -eq 1 ]]; then
        echo "no $extension sources found under $corpus_root/sources/$extension" >&2
        exit 1
    fi

    osascript "$script_dir/$exporter" "${exporter_args[@]}"

    local -a exported=("$staged_pdfs"/*.pdf)
    if [[ ! -e "${exported[0]}" ]]; then
        echo "no native PDFs exported for $extension sources" >&2
        exit 1
    fi
    cp "${exported[@]}" "$destination/"
    rm -rf "$stage"
}

export_format docx com.microsoft.Word export_word_pdfs.applescript "$stage_root/docx"
export_format pptx com.microsoft.Powerpoint export_powerpoint_pdfs.applescript "$stage_root/pptx"
export_format xlsx com.microsoft.Excel export_excel_pdfs.applescript "$stage_root/xlsx-sheets"

while IFS= read -r -d '' source; do
    stem="$(basename "${source%.xlsx}")"
    sheet_pdfs=("$stage_root/xlsx-sheets/$stem"-sheet-*.pdf)
    if [[ ! -e "${sheet_pdfs[0]}" ]]; then
        echo "no native Excel sheet PDFs found for $stem" >&2
        exit 1
    fi
    pdfunite "${sheet_pdfs[@]}" "$stage_root/xlsx/$stem.pdf"
done < <(find "$corpus_root/sources/xlsx" -maxdepth 1 -type f -name '*.xlsx' -print0 | sort -z)

{
    echo "exported_at=$(date -u +%Y-%m-%dT%H:%M:%SZ)"
    echo "macos=$(sw_vers -productVersion) build $(sw_vers -buildVersion)"
    for application in "Microsoft Word" "Microsoft PowerPoint" "Microsoft Excel"; do
        version="$("$plist_buddy" -c 'Print :CFBundleShortVersionString' "/Applications/$application.app/Contents/Info.plist")"
        echo "$application=$version"
    done
    find "$stage_root/docx" "$stage_root/pptx" "$stage_root/xlsx" -type f -name '*.pdf' -print0 \
        | sort -z \
        | xargs -0 shasum -a 256
} > "$stage_root/provenance.txt"

find "$stage_root/docx" "$stage_root/pptx" "$stage_root/xlsx" -type f -name '*.pdf' -print0 \
    | sort -z \
    | xargs -0 -n 1 pdfinfo >/dev/null

echo "staged native Office PDFs and provenance at $stage_root"
