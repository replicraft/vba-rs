' INV: vbal.5.4.5.12, stmt.get
' SOURCE: spec ms-vbal 5.4.5.12 declared type of the variable must not be Object
' XFAIL: wrong not implemented: static rejection of Object operands in Input/Put/Get
' ERROR: load
' GRANT: file-read
' GRANT: file-write
Public Sub Main()
    Dim o As Object
    Open "/g.bin" For Binary As #1
    Get #1, 1, o
End Sub
