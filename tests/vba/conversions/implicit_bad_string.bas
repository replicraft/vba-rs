' INV: expr.implicit-conv
' SOURCE: derived a non-numeric string assigned to a number is a type mismatch
' ERROR: 13
Public Sub Main()
    Dim n As Long
    n = "x"
End Sub
