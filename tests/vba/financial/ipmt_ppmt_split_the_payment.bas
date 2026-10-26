' INV: func.ipmt, func.ppmt, func.pmt
' SOURCE: derived the Learn IPmt and PPmt pages: the interest of a period is the rate times the outstanding balance and the principal is the rest of the payment; the schedule was evaluated outside the interpreter
' EXPECT: 86.07
' EXPECT: 4.59
' EXPECT: 81.47
' EXPECT: 86.07
Public Sub Main()
    Dim r As Double
    r = 0.06 / 12
    Debug.Print Format(Pmt(r, 12, -1000), "0.00")
    Debug.Print Format(IPmt(r, 2, 12, -1000), "0.00")
    Debug.Print Format(PPmt(r, 2, 12, -1000), "0.00")
    Debug.Print Format(IPmt(r, 2, 12, -1000) + PPmt(r, 2, 12, -1000), "0.00")
End Sub
