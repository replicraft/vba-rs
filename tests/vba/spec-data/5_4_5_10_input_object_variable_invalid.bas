' INV: vbal.5.4.5.10, stmt.input
' SOURCE: spec ms-vbal 5.4.5.10 declared type of an input variable must not be Object
' XFAIL: wrong not implemented: static rejection of Object operands in Input/Put/Get
' ERROR: load
' GRANT: file-read
' GRANT: file-write
Public Sub Main()
    Dim o As Object
    Open "/i.txt" For Input As #1
    Input #1, o
End Sub
