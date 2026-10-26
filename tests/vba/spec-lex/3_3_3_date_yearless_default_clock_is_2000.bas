' INV: vbal.3.3.3, lit.date
' SOURCE: spec ms-vbal 3.3.3 date tokens, a date literal without a year takes the current year (the year of the host clock at run time) (the default clock is 2000-01-01)
' CLOCK: 2000-01-01
' EXPECT: 2000
Public Sub Main()
    Debug.Print CStr(Year(#3/4#))
End Sub
