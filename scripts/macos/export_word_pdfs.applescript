on run argv
    if (count argv) < 3 or ((count argv) mod 2) is 0 then error "usage: output-directory id input-path [id input-path ...]"

    set outputDirectory to item 1 of argv
    set failures to {}
    do shell script "mkdir -p " & quoted form of outputDirectory

    tell application "Microsoft Word"
        launch
        repeat with argumentIndex from 2 to (count argv) by 2
            set outputId to item argumentIndex of argv
            set inputPath to item (argumentIndex + 1) of argv
            set outputPath to outputDirectory & "/" & outputId & ".pdf"
            set inputFile to my posixFile(inputPath)
            set openedDoc to missing value

            try
                with timeout of 120 seconds
                    open inputFile
                    -- `open` returns before Word registers the document, so
                    -- `active document` is `missing value` for the first few
                    -- seconds after a cold launch.
                    repeat 60 times
                        if (count of documents) > 0 then exit repeat
                        delay 1
                    end repeat
                    set openedDoc to active document
                    -- Word exports the review view: a document carrying
                    -- comments or tracked changes gets a grey markup strip down
                    -- the right margin, and the page body is squeezed to ~68%
                    -- width to make room for it. Converters target the final
                    -- document instead, so normalise to that view before
                    -- exporting — otherwise every element on every page reads
                    -- as displaced. Accepting revisions is also what makes the
                    -- ground truth show insertions and drop deletions, which is
                    -- the state a converter is expected to reproduce.
                    --
                    -- `print revisions` is a print-dialog property and does not
                    -- reach the "save as PDF" path, so the document has to be
                    -- edited in memory. Both statements are no-ops on a clean
                    -- document, and the file is closed with `saving no`, so
                    -- nothing changes on disk.
                    delete (every Word comment of openedDoc)
                    accept all revisions openedDoc
                    my createEmptyFile(outputPath)
                    save as openedDoc file name outputPath file format format PDF
                    my waitForNonEmptyFile(outputPath)
                    -- Exporting to PDF leaves Word holding a document object
                    -- that no longer answers `close`, so closing is best
                    -- effort; the `quit saving no` below is what actually
                    -- guarantees the next export starts clean. The PDF is
                    -- already on disk at this point, so a close failure must
                    -- not be reported as an export failure.
                    try
                        close openedDoc saving no
                    end try
                end timeout
            on error errorMessage number errorNumber
                try
                    close openedDoc saving no
                end try
                set end of failures to outputId & ": " & errorMessage & " (" & errorNumber & ")"
            end try
        end repeat
        -- A document left dirty by a failed export makes `quit` raise "user
        -- cancelled", which would mask the export failure we need to report.
        try
            quit saving no
        end try
    end tell

    if (count failures) > 0 then error my joinLines(failures)
end run

on posixFile(inputPath)
    return inputPath as POSIX file
end posixFile

on createEmptyFile(outputPath)
    do shell script ": > " & quoted form of outputPath
end createEmptyFile

on waitForNonEmptyFile(outputPath)
    repeat 120 times
        if (do shell script "test -s " & quoted form of outputPath & " && echo yes || true") is "yes" then return
        delay 1
    end repeat
    error "Word did not create a non-empty PDF"
end waitForNonEmptyFile

on joinLines(values)
    set AppleScript's text item delimiters to linefeed
    set joined to values as text
    set AppleScript's text item delimiters to ""
    return joined
end joinLines
