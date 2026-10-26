' INV: type.byte
' SOURCE: spec ms-vbal 5.5.1.2.1 numeric Let-coercion overflow
' ERROR: 6
Public Sub Main()
    Dim b As Byte
    b = -1
End Sub
