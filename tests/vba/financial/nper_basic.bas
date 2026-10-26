' INV: func.nper
' SOURCE: derived closed-form annuity formula from the function's documented definition; value rounded to 2 places
' EXPECT: 10.5886
' EXPECT: 10
Public Sub Main()
    Debug.Print CStr(Round(NPer(0.01, -100, 1000), 4))
    Debug.Print CStr(NPer(0, -100, 1000))
End Sub
