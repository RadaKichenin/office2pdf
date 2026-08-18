-- Report the recomputed standard row height of a batch of staged workbooks.
--
-- usage: osascript read_excel_row_heights.applescript stage-directory name [name ...]
--
-- One Excel launch covers the whole batch: a sweep of one workbook per font
-- variant costs a launch per reading otherwise. Each line of the result is
--     <file name><tab><standard height><tab><row 1 height>
-- and a workbook that fails to read raises, because a probe row silently
-- missing reads as "this factor does nothing" — the wrong answer (#810).
on run argv
    if (count argv) < 2 then error "usage: stage-directory name [name ...]"

    set stageDirectory to item 1 of argv
    -- Excel's dictionary shadows AppleScript's `tab` constant inside its
    -- `tell` block, where it coerces to the literal text "tab".
    set fieldSeparator to (ASCII character 9)
    set readings to {}
    set failures to {}

    tell application "Microsoft Excel"
        launch
        set display alerts to false
        repeat with argumentIndex from 2 to (count argv)
            set fileName to item argumentIndex of argv
            set openedWorkbook to missing value
            try
                with timeout of 180 seconds
                    open workbook workbook file name (stageDirectory & "/" & fileName) update links do not update links read only true ignore read only recommended true
                    set openedWorkbook to active workbook
                    -- `open workbook` answers from whatever document is already
                    -- frontmost when the open silently fails, which reports one
                    -- stale book's heights for the entire sweep.
                    if (name of openedWorkbook) is not fileName then
                        error "Excel answered from '" & (name of openedWorkbook) & "'"
                    end if
                    set sheetOne to worksheet 1 of openedWorkbook
                    set end of readings to fileName & fieldSeparator & (standard height of sheetOne as text) & fieldSeparator & (row height of row 1 of sheetOne as text)
                    close openedWorkbook saving no
                end timeout
            on error errorMessage number errorNumber
                if openedWorkbook is not missing value then
                    try
                        close openedWorkbook saving no
                    end try
                end if
                set end of failures to fileName & ": " & errorMessage & " (" & errorNumber & ")"
            end try
        end repeat
        quit
    end tell

    if (count failures) > 0 then error my joinLines(failures)
    return my joinLines(readings)
end run

on joinLines(values)
    set AppleScript's text item delimiters to linefeed
    set joined to values as text
    set AppleScript's text item delimiters to ""
    return joined
end joinLines
