' INV: vbal.6.1.2.11.1.1, func.asc
' SOURCE: spec ms-vbal 6.1.2.11.1.1 Asc of the null string raises error 5
' ERROR: 5
Public Sub Main()
    Dim n As Integer
    n = Asc("")
    Debug.Print "unreachable"
End Sub
