' INV: vbal.6.1.2.3.1.8, func.clng
' SOURCE: spec ms-vbal 6.1.2.3.1.8 CLng coercion
' EXPECT: r05=0
' EXPECT: r15=2
' EXPECT: r25=2
' EXPECT: max=2147483647
' EXPECT: min=-2147483648
' EXPECT: true=-1
' EXPECT: strExp=1000
' EXPECT: empty=0
' EXPECT: type=Long
' EXPECT: over=6
' EXPECT: under=6
' EXPECT: null=94
' EXPECT: text=13
Public Sub Main()
    Dim v As Variant
    Dim e As Variant
    Debug.Print "r05=" & CStr(CLng(0.5))
    Debug.Print "r15=" & CStr(CLng(1.5))
    Debug.Print "r25=" & CStr(CLng(2.5))
    Debug.Print "max=" & CStr(CLng(2147483647#))
    Debug.Print "min=" & CStr(CLng(-2147483648#))
    Debug.Print "true=" & CStr(CLng(True))
    Debug.Print "strExp=" & CStr(CLng("1e3"))
    Debug.Print "empty=" & CStr(CLng(e))
    Debug.Print "type=" & TypeName(CLng(1))
    On Error Resume Next
    Err.Clear: v = CLng(2147483648#): Debug.Print "over=" & Err.Number
    Err.Clear: v = CLng(-2147483649#): Debug.Print "under=" & Err.Number
    Err.Clear: v = CLng(Null): Debug.Print "null=" & Err.Number
    Err.Clear: v = CLng("zz"): Debug.Print "text=" & Err.Number
End Sub
