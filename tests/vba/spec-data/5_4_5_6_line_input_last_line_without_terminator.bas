' INV: vbal.5.4.5.6, stmt.line-input
' SOURCE: spec ms-vbal 5.4.5.6 end of file before a terminator yields the remaining text
' EXPECT: [a]
' EXPECT: [partial]
' EXPECT: eof=True
' GRANT: file-read
' GRANT: file-write
Public Sub Main()
    MkDir "/t"
    Dim s As String
    Open "/t/li.txt" For Output As #1
    Print #1, "a"
    Print #1, "partial";
    Close #1
    Open "/t/li.txt" For Input As #1
    Line Input #1, s
    Debug.Print "[" & s & "]"
    Line Input #1, s
    Debug.Print "[" & s & "]"
    Debug.Print "eof=" & CStr(EOF(1))
    Close #1
End Sub
