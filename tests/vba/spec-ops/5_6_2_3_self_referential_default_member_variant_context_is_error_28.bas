' INV: vbal.5.6.2.3, expr.default-member
' SOURCE: derived 5.6.2.3 allows an implementation-defined limit on default member recursion; where the target type is not known until run time the limit is enforced then, as the trappable error 28 (Out of stack space)
' METHOD: by construction: the chain never ends, any finite limit is exceeded
' EXPECT:  28 
'#MODULE Rec class
Public Property Get Me2() As Rec
Attribute Me2.VB_UserMemId = 0
    Set Me2 = Me
End Property
'#MODULE Module1 standard
Public Sub Main()
    Dim r As New Rec
    On Error Resume Next
    Debug.Print Len(r)
    Debug.Print Err.Number
End Sub
