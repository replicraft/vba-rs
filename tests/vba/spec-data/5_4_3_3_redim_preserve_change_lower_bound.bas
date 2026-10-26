' INV: vbal.5.4.3.3, stmt.redim
' SOURCE: spec ms-vbal 5.4.3.3 Preserve with changed lower bound is error 9
' ERROR: 9
Public Sub Main()
    Dim a() As Long
    ReDim a(1 To 3)
    ReDim Preserve a(2 To 3)
End Sub
