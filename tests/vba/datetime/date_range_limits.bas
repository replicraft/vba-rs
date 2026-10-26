' INV: type.date
' SOURCE: spec ms-vbal 2.1 Date value type (epoch and range) and 6.1.2.4.1.4 DateSerial year range
' EXPECT:  100 
' EXPECT:  9999  12  31 
' EXPECT:  2958465 
Public Sub Main()
    Dim d As Date
    d = DateSerial(100, 1, 1)
    Debug.Print Year(d)
    d = DateSerial(9999, 12, 31)
    Debug.Print Year(d); Month(d); Day(d)
    Debug.Print CDbl(d)
End Sub
