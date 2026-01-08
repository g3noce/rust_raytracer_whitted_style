use crate::aabb::Aabb;
use crate::objects::{Intersection, Object};
use crate::ray::Ray;
use crate::vec3::Vec3;
use std::cmp::Ordering;

#[derive(Clone, Copy, Debug)]
pub struct BvhNode {
    pub aabb: Aabb,
    pub left_first: u32,
    pub count: u32,
}

pub struct Bvh {
    pub nodes: Vec<BvhNode>,
    pub prim_indices: Vec<usize>,
}

pub struct BvhPrimitive {
    index: usize,
    aabb: Aabb,
    center: Vec3,
}

impl Bvh {
    pub fn build(objects: &[Object]) -> Self {
        let mut primitives: Vec<BvhPrimitive> = objects
            .iter()
            .enumerate()
            .map(|(i, obj)| {
                let aabb = obj.aabb();
                let center = (aabb.min + aabb.max) * 0.5;
                BvhPrimitive {
                    index: i,
                    aabb,
                    center,
                }
            })
            .collect();

        let mut nodes = Vec::with_capacity(objects.len() * 2);
        let mut prim_indices = vec![0; objects.len()];

        let root_node = BvhNode {
            aabb: Aabb::empty(),
            left_first: 0,
            count: 0,
        };
        nodes.push(root_node);

        Self::split(
            &mut nodes,
            &mut prim_indices,
            &mut primitives,
            0,
            0,
            objects.len(),
        );

        Bvh {
            nodes,
            prim_indices,
        }
    }

    fn split(
        nodes: &mut Vec<BvhNode>,
        global_indices: &mut [usize],
        primitives: &mut [BvhPrimitive],
        node_idx: usize,
        start: usize,
        count: usize,
    ) {
        let mut aabb = Aabb::empty();
        for i in 0..count {
            aabb = aabb.union(&primitives[start + i].aabb);
        }
        nodes[node_idx].aabb = aabb;
        nodes[node_idx].count = count as u32;
        nodes[node_idx].left_first = start as u32;

        if count <= 2 {
            for i in 0..count {
                global_indices[start + i] = primitives[start + i].index;
            }
            return;
        }

        let extent = aabb.max - aabb.min;
        let axis = if extent.x > extent.y && extent.x > extent.z {
            0
        } else if extent.y > extent.z {
            1
        } else {
            2
        };

        let slice = &mut primitives[start..start + count];
        slice.sort_by(|a, b| {
            let val_a = if axis == 0 {
                a.center.x
            } else if axis == 1 {
                a.center.y
            } else {
                a.center.z
            };
            let val_b = if axis == 0 {
                b.center.x
            } else if axis == 1 {
                b.center.y
            } else {
                b.center.z
            };
            val_a.partial_cmp(&val_b).unwrap_or(Ordering::Equal)
        });

        let mid = count / 2;
        let left_child_idx = nodes.len();
        let right_child_idx = left_child_idx + 1;

        nodes[node_idx].left_first = left_child_idx as u32;
        nodes[node_idx].count = 0;

        nodes.push(BvhNode {
            aabb: Aabb::empty(),
            left_first: 0,
            count: 0,
        });
        nodes.push(BvhNode {
            aabb: Aabb::empty(),
            left_first: 0,
            count: 0,
        });

        Self::split(
            nodes,
            global_indices,
            primitives,
            left_child_idx,
            start,
            mid,
        );
        Self::split(
            nodes,
            global_indices,
            primitives,
            right_child_idx,
            start + mid,
            count - mid,
        );
    }

    pub fn intersect(&self, ray: &Ray, objects: &[Object]) -> Option<Intersection> {
        let mut closest_t = f32::MAX;
        let mut closest_hit: Option<Intersection> = None;
        let mut stack = [0_usize; 64];
        let mut stack_ptr = 0;
        stack[0] = 0;

        while stack_ptr < 64 {
            let node_idx = stack[stack_ptr];
            let node = &self.nodes[node_idx];
            let dist_box = node.aabb.intersect(ray);

            if dist_box < closest_t {
                if node.count > 0 {
                    for i in 0..node.count {
                        let obj_idx = self.prim_indices[(node.left_first + i) as usize];
                        let obj = &objects[obj_idx];
                        if let Some((t, normal, mat)) = obj.intersect(ray) {
                            if t < closest_t {
                                closest_t = t;
                                closest_hit = Some(Intersection {
                                    point: ray.origin + t * ray.direction,
                                    normal,
                                    material: mat,
                                });
                            }
                        }
                    }
                    if stack_ptr == 0 {
                        break;
                    }
                    stack_ptr -= 1;
                } else {
                    let left_idx = node.left_first as usize;
                    let right_idx = left_idx + 1;
                    let node_l = &self.nodes[left_idx];
                    let node_r = &self.nodes[right_idx];
                    let dist_l = node_l.aabb.intersect(ray);
                    let dist_r = node_r.aabb.intersect(ray);

                    if dist_l != f32::MAX && dist_r != f32::MAX {
                        if dist_l < dist_r {
                            stack[stack_ptr] = right_idx;
                            stack_ptr += 1;
                            stack[stack_ptr] = left_idx;
                        } else {
                            stack[stack_ptr] = left_idx;
                            stack_ptr += 1;
                            stack[stack_ptr] = right_idx;
                        }
                    } else if dist_l != f32::MAX {
                        stack[stack_ptr] = left_idx;
                    } else if dist_r != f32::MAX {
                        stack[stack_ptr] = right_idx;
                    } else {
                        if stack_ptr == 0 {
                            break;
                        }
                        stack_ptr -= 1;
                    }
                }
            } else {
                if stack_ptr == 0 {
                    break;
                }
                stack_ptr -= 1;
            }
        }
        closest_hit
    }
}
