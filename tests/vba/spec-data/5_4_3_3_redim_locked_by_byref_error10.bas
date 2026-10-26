' INV: vbal.5.4.3.3, stmt.redim
' SOURCE: spec ms-vbal 5.4.3.3 redim of array locked by ByRef parameter is error 10
' XFAIL: wrong not implemented: error 10 for ReDim of an array locked by a ByRef argument
' ERROR: 10
Dim g() As Long
Private Sub Grow(ByRef x As Long)
    ReDim g(10)
End Sub
Public Sub Main()
    ReDim g(3)
    Grow g(1)
End Sub
