' INV: vbal.6.1.2.11.1.7, func.filter
' SOURCE: spec ms-vbal 6.1.2.11.1.7 Filter include/exclude, zero-based exact-size result
' EXPECT: inc=2 lb=0 ub=1
' EXPECT: 0=banana
' EXPECT: 1=bandana
' EXPECT: exc=1 lb=0 ub=0
' EXPECT: 0=apple
' EXPECT: t=String()
Public Sub Main()
    Dim src As Variant, r As Variant, i As Long
    src = Array("apple", "banana", "bandana")
    r = Filter(src, "an")
    Debug.Print "inc=" & CStr(UBound(r) - LBound(r) + 1) & " lb=" & CStr(LBound(r)) & " ub=" & CStr(UBound(r))
    For i = LBound(r) To UBound(r)
        Debug.Print CStr(i) & "=" & r(i)
    Next i
    r = Filter(src, "an", False)
    Debug.Print "exc=" & CStr(UBound(r) - LBound(r) + 1) & " lb=" & CStr(LBound(r)) & " ub=" & CStr(UBound(r))
    For i = LBound(r) To UBound(r)
        Debug.Print CStr(i) & "=" & r(i)
    Next i
    Debug.Print "t=" & TypeName(Filter(src, "an"))
End Sub
