' INV: vbal.3.3.3, lit.date
' SOURCE: spec ms-vbal 3.3.3 date tokens, a date literal without a year takes the current year (the year of the host clock at run time); February 29 does not exist in a year that is not a leap year, so evaluation fails at run time with type mismatch
' CLOCK: 2025-06-15
' ERROR: 13
Public Sub Main()
    Dim d As Date
    d = #2/29#
End Sub
