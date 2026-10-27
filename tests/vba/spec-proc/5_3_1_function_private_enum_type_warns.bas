' INV: vbal.5.3.1
' SOURCE: spec ms-vbal 5.3.1 declared type of function may not be a private enum (spec-strict: a warning by default, the program runs)
' EXPECT: ok
Private Enum Color
    Red = 1
End Enum
Public Function F() As Color
    F = Red
End Function
Public Sub Main()
    Debug.Print "ok"
End Sub
