' INV: vbal.5.4.5.8, stmt.print
' SOURCE: spec ms-vbal 5.4.5.8 Date output is its string coercion, no added spaces
' EXPECT: same=True
' GRANT: file-read
' GRANT: file-write
Public Sub Main()
    MkDir "/t"
    Dim s As String, d As Date
    d = DateSerial(2020, 1, 2)
    Open "/t/p.txt" For Output As #1
    Print #1, d
    Close #1
    Open "/t/p.txt" For Input As #1
    Line Input #1, s
    Close #1
    Debug.Print "same=" & CStr(s = CStr(d))
End Sub
