' INV: vbal.6.1.2.11.1.2
' SOURCE: spec ms-vbal 6.1.2.11.1.2 AscB of the null string raises error 5
' ERROR: 5
Public Sub Main()
    Dim n As Integer
    n = AscB("")
    Debug.Print "unreachable"
End Sub
