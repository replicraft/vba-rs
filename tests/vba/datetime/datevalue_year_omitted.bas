' INV: func.datevalue, func.isdate, vbal.6.1.2.4.1.5
' SOURCE: doc https://learn.microsoft.com/en-us/office/vba/language/reference/user-interface-help/datevalue-function if the year part of the string is omitted, DateValue uses the current year from the computer's system date; IsDate follows the same parsing
' CLOCK: 2024-06-15 12:30:00
' EXPECT: 12/30/2024
' EXPECT: 12/30/2024
' EXPECT: 12/30/2024
' EXPECT: 1/5/2024
' EXPECT: 12/1/2025
' EXPECT: True
' EXPECT: True
' EXPECT: False
' EXPECT: 13
Public Sub Main()
    Debug.Print CStr(DateValue("December 30"))
    Debug.Print CStr(DateValue("30 Dec"))
    Debug.Print CStr(DateValue("12/30"))
    Debug.Print CStr(DateValue("Jan 5 6:00 PM"))
    Debug.Print CStr(DateValue("Dec 2025"))
    Debug.Print CStr(IsDate("Jan 5"))
    Debug.Print CStr(IsDate("12/30"))
    Debug.Print CStr(IsDate("Feb 30"))
    Dim v As Variant
    On Error Resume Next
    v = DateValue("13/45")
    Debug.Print CStr(Err.Number)
End Sub
