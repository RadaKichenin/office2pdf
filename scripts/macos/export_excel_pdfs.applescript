on run argv
    if (count argv) < 3 or ((count argv) mod 2) is 0 then error "usage: output-directory id input-path [id input-path ...]"

    set outputDirectory to item 1 of argv
    set failures to {}
    do shell script "mkdir -p " & quoted form of outputDirectory

    tell application "Microsoft Excel"
        launch
        set display alerts to false
        repeat with argumentIndex from 2 to (count argv) by 2
            set outputId to item argumentIndex of argv
            set inputPath to item (argumentIndex + 1) of argv
            set openedWorkbook to missing value

            try
                my removePreviousSheetPdfs(outputDirectory, outputId)
                with timeout of 120 seconds
                    open workbook workbook file name inputPath update links do not update links read only true ignore read only recommended true
                    set openedWorkbook to active workbook
                    set visibleSheetCount to 0

                    repeat with sheetIndex from 1 to (count worksheets of openedWorkbook)
                        set currentSheet to worksheet sheetIndex of openedWorkbook
                        if visible of currentSheet is sheet visible then
                            set visibleSheetCount to visibleSheetCount + 1
                            set outputPath to outputDirectory & "/" & outputId & "-sheet-" & my paddedIndex(sheetIndex) & ".pdf"
                            set outputFile to my createEmptyFile(outputPath)
                            -- Excel's PDF save as always exports every visible sheet of the
                            -- workbook, so hiding the others is the only way to scope the
                            -- export to just this worksheet.
                            set hiddenSheetNames to my hideOtherVisibleSheets(openedWorkbook, name of currentSheet)
                            save as currentSheet filename outputFile file format PDF file format
                            my waitForNonEmptyFile(outputPath)
                            my restoreSheetVisibility(openedWorkbook, hiddenSheetNames)
                        end if
                    end repeat

                    if visibleSheetCount is 0 then error "workbook has no visible worksheets"
                    close openedWorkbook saving no
                end timeout
            on error errorMessage number errorNumber
                if openedWorkbook is not missing value then
                    try
                        close openedWorkbook saving no
                    end try
                end if
                set end of failures to outputId & ": " & errorMessage & " (" & errorNumber & ")"
            end try
        end repeat
        quit
    end tell

    if (count failures) > 0 then error my joinLines(failures)
end run

on hideOtherVisibleSheets(openedWorkbook, currentSheetName)
    set hiddenSheetNames to {}
    tell application "Microsoft Excel"
        -- Iterate every sheet (not just worksheets) so chart sheets are hidden
        -- too; match by name because object-specifier equality is unreliable.
        -- The explicit `get` materializes the list before iterating: a bare
        -- `repeat with x in (every sheet of wb)` re-resolves `item N of
        -- (every sheet ...)` per iteration, which Excel rejects with
        -- parameter error -50 even though `get every sheet` succeeds.
        repeat with candidateSheet in (get every sheet of openedWorkbook)
            if (name of candidateSheet) is not currentSheetName then
                if visible of candidateSheet is sheet visible then
                    set visible of candidateSheet to sheet hidden
                    set end of hiddenSheetNames to (name of candidateSheet)
                end if
            end if
        end repeat
    end tell
    return hiddenSheetNames
end hideOtherVisibleSheets

on restoreSheetVisibility(openedWorkbook, hiddenSheetNames)
    tell application "Microsoft Excel"
        repeat with hiddenSheetName in hiddenSheetNames
            set visible of sheet (hiddenSheetName as text) of openedWorkbook to sheet visible
        end repeat
    end tell
end restoreSheetVisibility

on removePreviousSheetPdfs(outputDirectory, outputId)
    set filePattern to outputId & "-sheet-*.pdf"
    do shell script "find " & quoted form of outputDirectory & " -maxdepth 1 -type f -name " & quoted form of filePattern & " -delete"
end removePreviousSheetPdfs

on paddedIndex(sheetIndex)
    return do shell script "printf '%04d' " & sheetIndex
end paddedIndex

on createEmptyFile(outputPath)
    do shell script ": > " & quoted form of outputPath
    return outputPath as POSIX file
end createEmptyFile

on waitForNonEmptyFile(outputPath)
    repeat 120 times
        if (do shell script "test -s " & quoted form of outputPath & " && echo yes || true") is "yes" then return
        delay 1
    end repeat
    error "Excel did not create a non-empty PDF"
end waitForNonEmptyFile

on joinLines(values)
    set AppleScript's text item delimiters to linefeed
    set joined to values as text
    set AppleScript's text item delimiters to ""
    return joined
end joinLines
