' INV: vbal.5.4.5.11, stmt.put
' SOURCE: spec ms-vbal 5.4.5.11 data expression must not have declared type Object
' XFAIL: wrong not implemented: static rejection of Object operands in Input/Put/Get
' ERROR: load
' GRANT: file-read
' GRANT: file-write
Public Sub Main()
    Dim o As Object
    Open "/g.bin" For Binary As #1
    Put #1, 1, o
End Sub
