' INV: func.format
' SOURCE: doc https://learn.microsoft.com/en-us/office/vba/language/reference/user-interface-help/format-function the AM/PM, am/pm, A/P, a/p and AMPM characters are date/time format characters, so a format made of only a designator is a date format
' EXPECT: PM
' EXPECT: pm
' EXPECT: P
' EXPECT: p
' EXPECT: PM
' EXPECT: AM
Public Sub Main()
    Dim t As Date
    t = #1/2/2024 3:04:05 PM#
    Debug.Print Format(t, "AM/PM")
    Debug.Print Format(t, "am/pm")
    Debug.Print Format(t, "A/P")
    Debug.Print Format(t, "a/p")
    Debug.Print Format(t, "AMPM")
    Debug.Print Format(#1/2/2024 9:04:05 AM#, "AM/PM")
End Sub
