' INV: vbal.3.3.3, lit.date
' SOURCE: spec ms-vbal 3.3.3 date tokens, numeric value is days from the 1899-12-30 epoch plus the fraction of the day
' EXPECT: 0
' EXPECT: 1
' EXPECT: 1.25
' EXPECT: 0.5625
' EXPECT: 0.5
Public Sub Main()
    Debug.Print CStr(CDbl(#12/30/1899#))
    Debug.Print CStr(CDbl(#12/31/1899#))
    Debug.Print CStr(CDbl(#12/31/1899 6:00 AM#))
    Debug.Print CStr(CDbl(#13:30#))
    Debug.Print CStr(CDbl(#12:00 PM#))
End Sub
