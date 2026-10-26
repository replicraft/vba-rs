' INV: stmt.date, stmt.time
' SOURCE: derived the Date statement is documented; the default host refuses to change the system clock with error 70
' ERROR: 70
Public Sub Main()
    Date = DateSerial(2001, 2, 3)
    Debug.Print Year(Date)
End Sub
