' INV: vbal.6.1.2.3.1.12, func.cstr
' SOURCE: spec ms-vbal 6.1.2.3.1.12 CStr string coercion formats
' EXPECT: err5=Error 5
' EXPECT: err0=Error 0
' EXPECT: true=True
' EXPECT: false=False
' EXPECT: zero=0
' EXPECT: dbl=1.5
' EXPECT: neg=-123.45
' EXPECT: dblzero=0
' EXPECT: empty=[]
' EXPECT: str=abc
' EXPECT: ll=123456789012345678
' EXPECT: cur=1.5
' EXPECT: dbl15=123456789012345
' EXPECT: sci16=1E+16
' EXPECT: sci15=1E+15
' EXPECT: sciNeg=-1.5E+20
' EXPECT: type=String
Public Sub Main()
    Dim e As Variant
    Debug.Print "err5=" & CStr(CVErr(5))
    Debug.Print "err0=" & CStr(CVErr(0))
    Debug.Print "true=" & CStr(True)
    Debug.Print "false=" & CStr(False)
    Debug.Print "zero=" & CStr(0)
    Debug.Print "dbl=" & CStr(1.5)
    Debug.Print "neg=" & CStr(-123.45)
    Debug.Print "dblzero=" & CStr(0#)
    Debug.Print "empty=[" & CStr(e) & "]"
    Debug.Print "str=" & CStr("abc")
    Debug.Print "ll=" & CStr(CLngLng(123456789012345678^))
    Debug.Print "cur=" & CStr(CCur(1.5))
    Debug.Print "dbl15=" & CStr(123456789012345#)
    Debug.Print "sci16=" & CStr(1E+16)
    Debug.Print "sci15=" & CStr(1E+15)
    Debug.Print "sciNeg=" & CStr(-1.5E+20)
    Debug.Print "type=" & TypeName(CStr(1))
End Sub
