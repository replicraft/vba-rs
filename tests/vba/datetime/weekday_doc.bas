' INV: func.weekday
' SOURCE: doc https://learn.microsoft.com/en-us/office/vba/language/reference/user-interface-help/weekday-function
' EXPECT:  4 
Public Sub Main()
    Dim d As Date
    d = #February 12, 1969#
    Debug.Print Weekday(d)
End Sub
