' INV: func.year, type.date
' SOURCE: spec ms-vbal 6.1.2.4.1.14 Year range and 2.1 Date value type epoch
' EXPECT:  100  9999  1899 
Public Sub Main()
    Debug.Print Year(DateSerial(100, 1, 1)); Year(DateSerial(9999, 12, 31)); Year(0)
End Sub
