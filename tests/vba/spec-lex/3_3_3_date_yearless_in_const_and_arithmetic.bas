' INV: vbal.3.3.3, lit.date
' SOURCE: spec ms-vbal 3.3.3 date tokens, a date literal without a year takes the current year (the year of the host clock at run time); it is not folded at analysis time
' CLOCK: 2026-01-01
' EXPECT: 2026-3-5
' EXPECT: 2026-3-4
Const D As Date = #3/4#
Public Sub Main()
    Dim n As Date
    n = #3/4# + 1
    Debug.Print Year(n) & "-" & Month(n) & "-" & Day(n)
    Debug.Print Year(D) & "-" & Month(D) & "-" & Day(D)
End Sub
