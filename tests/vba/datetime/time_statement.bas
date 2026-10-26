' INV: stmt.time
' SOURCE: doc https://learn.microsoft.com/en-us/office/vba/language/reference/user-interface-help/time-statement
' EXPECT:  10 
' XFAIL: unsupported setting the system clock is not provided by the stdlib host
Public Sub Main()
    Time = TimeSerial(10, 0, 0)
    Debug.Print Hour(Time)
End Sub
