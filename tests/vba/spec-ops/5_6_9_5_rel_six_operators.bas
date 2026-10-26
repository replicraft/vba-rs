' INV: vbal.5.6.9.5.1, vbal.5.6.9.5.3, vbal.5.6.9.5.4, vbal.5.6.9.5.5, vbal.5.6.9.5.6, op.eq, op.ne, op.lt, op.gt, op.le, op.ge
' SOURCE: spec ms-vbal 5.6.9.5 relational operators truth tables for = <> < > <= >=
' EXPECT: FTTFFT
' EXPECT: TFFFTT
' EXPECT: FTFTTF
Private Function B(ByVal x As Boolean) As String
    If x Then B = "T" Else B = "F"
End Function
Public Sub Main()
    Dim a As Long, c As Long
    a = 1: c = 2
    Debug.Print B(a = c) & B(a <> c) & B(a < c) & B(a > c) & B(a >= c) & B(a <= c)
    a = 2: c = 2
    Debug.Print B(a = c) & B(a <> c) & B(a < c) & B(a > c) & B(a >= c) & B(a <= c)
    a = 3: c = 2
    Debug.Print B(a = c) & B(a <> c) & B(a < c) & B(a > c) & B(a >= c) & B(a <= c)
End Sub
