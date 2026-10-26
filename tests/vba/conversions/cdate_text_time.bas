' INV: func.cdate
' SOURCE: doc https://learn.microsoft.com/en-us/office/vba/language/concepts/getting-started/type-conversion-functions
' EXPECT:  16  35  47 
Public Sub Main()
    Dim d As Date
    d = CDate("4:35:47 PM")
    Debug.Print Hour(d); Minute(d); Second(d)
End Sub
