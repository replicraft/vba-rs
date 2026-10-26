' INV: func.rate
' SOURCE: derived Rate solves the annuity equation iteratively; checked by substituting the result back
' EXPECT: 0.0136
' EXPECT: 1100
Public Sub Main()
    Dim r As Double
    r = Rate(12, -100, 1100)
    Debug.Print CStr(Round(r, 4))
    Debug.Print CStr(Round(PV(r, 12, -100), 2))
End Sub
