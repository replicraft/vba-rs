' INV: vbal.5.4.3.8, stmt.let
' SOURCE: spec ms-vbal 5.4.3.8 assigning the enclosing function sets its return value
' EXPECT: 7
Private Function F() As Long
    F = 7
End Function
Public Sub Main()
    Debug.Print CStr(F())
End Sub
