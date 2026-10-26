' INV: func.cbool
' SOURCE: derived a non-numeric string is a type mismatch
' ERROR: 13
Public Sub Main()
    Debug.Print CBool("abc")
End Sub
