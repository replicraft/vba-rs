' INV: stmt.redim
' SOURCE: spec ms-vbal 5.4.3.3 ReDim Preserve changing a non-last dimension raises error 9
' ERROR: 9
Public Sub Main()
    Dim m() As Long
    ReDim m(1 To 2, 1 To 2)
    ReDim Preserve m(1 To 3, 1 To 2)
End Sub
