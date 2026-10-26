' INV: vbal.5.4.3.3, stmt.redim
' SOURCE: spec ms-vbal 5.4.3.3 Preserve changing the number of dimensions is error 9
' ERROR: 9
Public Sub Main()
    Dim a() As Long
    ReDim a(1)
    ReDim Preserve a(1, 1)
End Sub
