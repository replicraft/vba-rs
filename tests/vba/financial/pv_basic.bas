' INV: func.pv
' SOURCE: derived closed-form annuity formula from the function's documented definition; value rounded to 2 places
' EXPECT: 19999.75
' EXPECT: 1000
Public Sub Main()
    Debug.Print CStr(Round(PV(0.05 / 12, 60, -377.42), 2))
    Debug.Print CStr(Round(PV(0, 10, -100), 2))
End Sub
