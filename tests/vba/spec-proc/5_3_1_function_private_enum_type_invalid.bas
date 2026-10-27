' INV: vbal.5.3.1
' SOURCE: spec ms-vbal 5.3.1 declared type of function may not be a private enum
' STRICT_SPEC: true
' ERROR: load
Private Enum Color
    Red = 1
End Enum
Public Function F() As Color
    F = Red
End Function
Public Sub Main()
End Sub
