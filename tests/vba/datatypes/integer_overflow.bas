' INV: type.integer
' SOURCE: spec ms-vbal 5.6.9.3 arithmetic overflow
' ERROR: 6
Public Sub Main()
    Dim i As Integer
    i = 32767
    i = i + 1
End Sub
