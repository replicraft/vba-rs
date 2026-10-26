' INV: vbal.5.4.5.1, stmt.open
' SOURCE: spec ms-vbal 5.4.5.1 Append adds data at the end
' EXPECT: first
' EXPECT: second
' GRANT: file-read
' GRANT: file-write
Public Sub Main()
    MkDir "/t"
    Dim s As String
    Open "/t/ap.txt" For Output As #1
    Print #1, "first"
    Close #1
    Open "/t/ap.txt" For Append As #1
    Print #1, "second"
    Close #1
    Open "/t/ap.txt" For Input As #1
    Do While Not EOF(1)
        Line Input #1, s
        Debug.Print s
    Loop
    Close #1
End Sub
