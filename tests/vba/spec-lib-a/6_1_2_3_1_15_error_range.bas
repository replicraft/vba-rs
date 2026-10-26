' INV: vbal.6.1.2.3.1.15, func.error
' SOURCE: spec ms-vbal 6.1.2.3.1.15 Error/Error$ argument range
' XFAIL: wrong not implemented: Error$ with no argument and the 0..65535 range
' EXPECT: over=6
' EXPECT: overDollar=6
' EXPECT: max=0
' EXPECT: neg=0
' EXPECT: text=13
Public Sub Main()
    Dim v As Variant
    On Error Resume Next
    Err.Clear: v = Error(65536): Debug.Print "over=" & Err.Number
    Err.Clear: v = Error$(65536): Debug.Print "overDollar=" & Err.Number
    Err.Clear: v = Error$(65535): Debug.Print "max=" & Err.Number
    Err.Clear: v = Error$(-1): Debug.Print "neg=" & Err.Number
    Err.Clear: v = Error$("abc"): Debug.Print "text=" & Err.Number
End Sub
