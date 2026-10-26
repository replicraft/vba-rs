' INV: func.asc
' SOURCE: spec ms-vbal 6.1.2.11.1.1 Asc null string raises error 5
' ERROR: 5
Public Sub Main()
    Debug.Print Asc("")
End Sub
