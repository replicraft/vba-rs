' INV: func.left
' SOURCE: derived negative length is error 5
' ERROR: 5
Public Sub Main()
    Debug.Print Left("abc", -1)
End Sub
