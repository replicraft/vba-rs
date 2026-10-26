' INV: vbal.5.4.3.1, stmt.dim
' SOURCE: spec ms-vbal 5.4.3.1 variable must not have the containing function's name
' XFAIL: wrong pending
' ERROR: load
Private Function F() As Integer
    Dim F As Integer
    F = 1
End Function
Public Sub Main()
    Debug.Print F()
End Sub
