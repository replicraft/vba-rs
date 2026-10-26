' INV: vbal.5.4.5.10, stmt.input
' SOURCE: spec ms-vbal 5.4.5.10 Date input not starting with # is error 6
' ERROR: 6
' GRANT: file-read
' GRANT: file-write
Public Sub Main()
    MkDir "/t"
    Dim d As Date
    Open "/t/i.txt" For Output As #1
    Print #1, "2020-01-02"
    Close #1
    Open "/t/i.txt" For Input As #1
    Input #1, d
End Sub
