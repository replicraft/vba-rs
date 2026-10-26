' INV: func.pmt
' SOURCE: derived the Learn Pmt page: payment = -(pv(1+r)^n + fv) r / ((1 + r type)((1+r)^n - 1)); loan of 20000 at 6 percent a year over 60 months; the factors were evaluated outside the interpreter
' EXPECT: 386.66
' EXPECT: 384.73
' EXPECT: 1281.98
Public Sub Main()
    Debug.Print Format(Pmt(0.06 / 12, 60, -20000), "0.00")
    Debug.Print Format(Pmt(0.06 / 12, 60, -20000, 0, 1), "0.00")
    Debug.Print Format(Pmt(0.05 / 12, 360, -250000, 50000), "0.00")
End Sub
