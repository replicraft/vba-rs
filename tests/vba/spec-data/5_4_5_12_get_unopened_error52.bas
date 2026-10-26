' INV: vbal.5.4.5.12, stmt.get
' SOURCE: spec ms-vbal 5.4.5.12 Get from a number that is not open is error 52
' ERROR: 52
' GRANT: file-read
' GRANT: file-write
Public Sub Main()
    Dim n As Long
    Get #5, 1, n
End Sub
