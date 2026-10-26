' INV: vbal.6.1.2.3.1.19, func.oct
' SOURCE: spec ms-vbal 6.1.2.3.1.19 Oct values, negative widths 6/11/22 characters
' EXPECT: zero=0
' EXPECT: seven=7
' EXPECT: empty=0
' EXPECT: minus1=177777
' EXPECT: minus1Len=6
' EXPECT: minus32767=100001
' EXPECT: minus32768=37777700000
' EXPECT: minus32768Len=11
' EXPECT: ll=1777777777757777777777
' EXPECT: llLen=22
' EXPECT: type=String
' EXPECT: dollarType=String
Public Sub Main()
    Dim e As Variant
    Debug.Print "zero=" & Oct(0)
    Debug.Print "seven=" & Oct(7)
    Debug.Print "empty=" & Oct(e)
    Debug.Print "minus1=" & Oct(-1)
    Debug.Print "minus1Len=" & CStr(Len(Oct(-1)))
    Debug.Print "minus32767=" & Oct(-32767)
    Debug.Print "minus32768=" & Oct(CLng(-32768))
    Debug.Print "minus32768Len=" & CStr(Len(Oct(CLng(-32768))))
    Debug.Print "ll=" & Oct(CLngLng(-2147483649#))
    Debug.Print "llLen=" & CStr(Len(Oct(CLngLng(-2147483649#))))
    Debug.Print "type=" & TypeName(Oct(1))
    Debug.Print "dollarType=" & TypeName(Oct$(1))
End Sub
