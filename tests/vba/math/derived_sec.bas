' INV: derived.sec, func.cos
' SOURCE: doc https://learn.microsoft.com/en-us/office/vba/language/reference/user-interface-help/derived-math-functions
' EXPECT: 1
' EXPECT: 2
Public Sub Main()
    Debug.Print CStr(Round(1 / Cos(0), 10))
    Debug.Print CStr(Round(1 / Cos(1.0471975511966), 6))
End Sub
