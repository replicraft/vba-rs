' INV: type.byte
' SOURCE: spec ms-vbal 2.1 Byte value type
' EXPECT:  255 
' EXPECT:  0 
Public Sub Main()
    Dim b As Byte
    b = 255
    Debug.Print b
    b = 0
    Debug.Print b
End Sub
