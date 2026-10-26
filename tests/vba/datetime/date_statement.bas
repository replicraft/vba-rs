' INV: stmt.date, stmt.time
' SOURCE: doc https://learn.microsoft.com/en-us/office/vba/language/reference/user-interface-help/date-statement
' EXPECT:  2001 
' XFAIL: unsupported setting the system clock is not provided by the stdlib host
Public Sub Main()
    Date = DateSerial(2001, 2, 3)
    Debug.Print Year(Date)
End Sub
