' INV: vbal.5.4.5.10, stmt.input
' SOURCE: spec ms-vbal 5.4.5.10 numeric characters are read and coerced to the declared type
' EXPECT: 42
' EXPECT: -1.5
' GRANT: file-read
' GRANT: file-write
Public Sub Main()
    MkDir "/t"
    Dim n As Long, d As Double
    Open "/t/i.txt" For Output As #1
    Print #1, "42"
    Close #1
    Open "/t/i.txt" For Input As #1
    Input #1, n
    Close #1
    Open "/t/i2.txt" For Output As #1
    Print #1, "-1.5"
    Close #1
    Open "/t/i2.txt" For Input As #1
    Input #1, d
    Close #1
    Debug.Print CStr(n)
    Debug.Print CStr(d)
End Sub
