' INV: vbal.5.6.2.3, expr.default-member
' SOURCE: derived 5.6.2.3 allows an implementation-defined limit on default member recursion; a default member that returns its own class never reaches a value, so using it as a operand ends at run time in the trappable error 28 and not in an endless descent
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
    Debug.Print r + 1
    Debug.Print Err.Number
End Sub
