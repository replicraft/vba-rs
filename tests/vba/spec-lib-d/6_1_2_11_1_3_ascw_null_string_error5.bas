' INV: vbal.6.1.2.11.1.3
' SOURCE: spec ms-vbal 6.1.2.11.1.3 AscW of the null string raises error 5
' ERROR: 5
Public Sub Main()
    Dim n As Integer
    n = AscW("")
    Debug.Print "unreachable"
End Sub
