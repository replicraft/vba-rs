' INV: func.asc
' SOURCE: derived Asc of an empty string is error 5
' ERROR: 5
Public Sub Main()
    Debug.Print Asc("")
End Sub
