' INV: func.fv
' SOURCE: derived closed-form annuity formula from the function's documented definition; value rounded to 2 places
' EXPECT: 15528.23
' EXPECT: 1000
' EXPECT: 2395.08
Public Sub Main()
    Debug.Print CStr(Round(FV(0.05 / 12, 120, -100), 2))
    Debug.Print CStr(Round(FV(0, 10, -100), 2))
    Debug.Print CStr(Round(FV(0.01, 12, -100, -1000), 2))
End Sub
