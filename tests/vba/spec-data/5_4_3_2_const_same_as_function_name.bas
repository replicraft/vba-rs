' INV: vbal.5.4.3.2, stmt.const
' SOURCE: spec ms-vbal 5.4.3.2 constant must not have the containing function's name
' ERROR: load
Private Function F() As Integer
    Const F = 1
    F = 2
End Function
Public Sub Main()
    Debug.Print F()
End Sub
