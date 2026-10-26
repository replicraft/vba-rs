' INV: func.weekday
' SOURCE: doc https://learn.microsoft.com/en-us/office/vba/language/reference/user-interface-help/weekday-function
' EXPECT:  3  4  1  7 
Public Sub Main()
    Dim d As Date
    d = #February 12, 1969#
    Debug.Print Weekday(d, vbMonday); Weekday(d, vbSunday); Weekday(d, vbWednesday); Weekday(d, vbThursday)
End Sub
