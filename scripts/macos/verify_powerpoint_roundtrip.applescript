-- Acceptance probe for lossless round-trip artifacts (issue #746): open each
-- file in Microsoft PowerPoint and report its slide count. A file that
-- triggers a repair prompt blocks `open` until the timeout, which is recorded
-- as a failure — so a clean report means PowerPoint accepted the file as-is.
--
-- usage: osascript verify_powerpoint_roundtrip.applescript input.pptx [more.pptx ...]
on run argv
    if (count argv) < 1 then error "usage: input-path [input-path ...]"

    set failures to {}
    set reportLines to {}

    tell application "Microsoft PowerPoint"
        launch
        repeat with inputIndex from 1 to (count argv)
            set inputPath to item inputIndex of argv
            set openedPresentation to missing value
            try
                with timeout of 60 seconds
                    open my posixFile(inputPath)
                    set openedPresentation to active presentation
                    set slideCount to count of slides of openedPresentation
                    close openedPresentation saving no
                    set end of reportLines to inputPath & ": opened cleanly, " & slideCount & " slides"
                end timeout
            on error errorMessage number errorNumber
                if openedPresentation is not missing value then
                    try
                        close openedPresentation saving no
                    end try
                end if
                set end of failures to inputPath & ": " & errorMessage & " (" & errorNumber & ")"
            end try
        end repeat
        quit
    end tell

    if (count failures) > 0 then error my joinLines(failures)
    return my joinLines(reportLines)
end run

on posixFile(inputPath)
    return inputPath as POSIX file
end posixFile

on joinLines(values)
    set AppleScript's text item delimiters to linefeed
    set joined to values as text
    set AppleScript's text item delimiters to ""
    return joined
end joinLines
