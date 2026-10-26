' INV: func.mid
' SOURCE: derived start must be at least 1; otherwise error 5 (Invalid procedure call)
' ERROR: 5
Public Sub Main()
    Debug.Print Mid("abc", 0)
End Sub
