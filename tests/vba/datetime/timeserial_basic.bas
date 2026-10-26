' INV: func.timeserial
' SOURCE: spec ms-vbal 6.1.2.4.1.11 TimeSerial
' EXPECT:  13  5  9 
' EXPECT:  1  30 
Public Sub Main()
    Dim t As Date
    t = TimeSerial(13, 5, 9)
    Debug.Print Hour(t); Minute(t); Second(t)
    t = TimeSerial(0, 90, 0)
    Debug.Print Hour(t); Minute(t)
End Sub
