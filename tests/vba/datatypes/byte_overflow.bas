' INV: type.byte
' SOURCE: spec ms-vbal 5.6.9.3 arithmetic overflow
' ERROR: 6
Public Sub Main()
    Dim b As Byte
    b = 255
    b = b + 1
End Sub
