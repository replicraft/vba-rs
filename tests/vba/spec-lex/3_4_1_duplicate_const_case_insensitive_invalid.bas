' INV: vbal.3.4.1, cc.const
' SOURCE: spec ms-vbal 3.4.1 conditional compilation const directive, names are compared case-insensitively, and a duplicate in an excluded block still counts
' XFAIL: wrong pending
' ERROR: load
#Const Flag = 1
#If False Then
#Const FLAG = 2
#End If
Public Sub Main()
End Sub
