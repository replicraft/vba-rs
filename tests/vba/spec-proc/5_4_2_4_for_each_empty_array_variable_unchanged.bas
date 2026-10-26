' INV: vbal.5.4.2.4
' SOURCE: spec ms-vbal 5.4.2.4 For Each over an array without elements completes immediately
' EXPECT: n=0 v=keep
Public Sub Main()
    Dim a As Variant
    a = Array()
    Dim v As Variant, n As Long
    v = "keep"
    For Each v In a
        n = n + 1
    Next v
    Debug.Print "n=" & CStr(n) & " v=" & CStr(v)
End Sub
