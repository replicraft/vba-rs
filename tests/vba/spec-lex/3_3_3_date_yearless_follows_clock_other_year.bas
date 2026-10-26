' INV: vbal.3.3.3, lit.date
' SOURCE: spec ms-vbal 3.3.3 date tokens, a date literal without a year takes the current year (the year of the host clock at run time)
' CLOCK: 2031-01-01
' EXPECT: 2031
' EXPECT: 2031-12-25
Public Sub Main()
    Debug.Print CStr(Year(#12/25#))
    Debug.Print Format(#Dec 25#, "yyyy-mm-dd")
End Sub
