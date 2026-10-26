' INV: stmt.time
' SOURCE: derived the Time statement is documented; the default host refuses to change the system clock with error 70
' ERROR: 70
Public Sub Main()
    Time = TimeSerial(10, 0, 0)
    Debug.Print Hour(Time)
End Sub
