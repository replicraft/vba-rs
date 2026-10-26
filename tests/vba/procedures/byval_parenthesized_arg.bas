' INV: decl.byref-byval
' SOURCE: unverified author's knowledge of VBA semantics; no document consulted
' EXPECT:  1 
' EXPECT:  2 
Sub Inc(n As Long)
    n = n + 1
End Sub
Public Sub Main()
    Dim x As Long
    x = 1
    Inc (x)
    Debug.Print x
    Call Inc(x)
    Debug.Print x
End Sub
